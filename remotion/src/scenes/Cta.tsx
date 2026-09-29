import React from 'react';
import {AbsoluteFill, interpolate, spring, useCurrentFrame, useVideoConfig} from 'remotion';
import {CONFIG} from '../config';
import {C, F} from '../theme';
import {BlurWords, Card, Logo, Pill, Wordmark, clamp, useFadeUp} from '../ui';

const STATS = [
  {value: 3, prefix: '<', suffix: 'ms', label: 'local decision p50', count: false},
  {value: 13, prefix: '', suffix: '', label: 'versioned hard-deny rules', count: true},
  {value: 240, prefix: '', suffix: '', label: 'eval records', count: true},
  {value: 174, prefix: '', suffix: '+', label: 'tests green across the stack', count: true},
];

const StatTile: React.FC<{s: (typeof STATS)[number]; start: number}> = ({s, start}) => {
  const frame = useCurrentFrame();
  const fade = useFadeUp(start, 16, 20);
  const p = interpolate(frame, [start, start + 40], [0, 1], clamp);
  const shown = s.count ? Math.round(s.value * p) : s.value;
  return (
    <div style={{...fade, flex: 1}}>
      <Card style={{padding: '26px 28px'}}>
        <div style={{fontFamily: F.mono, fontSize: 68, fontWeight: 700, color: C.brandStrong, letterSpacing: '-0.03em'}}>
          {s.prefix}
          {shown}
          {s.suffix}
        </div>
        <div style={{fontSize: 22, color: C.muted, marginTop: 4}}>{s.label}</div>
      </Card>
    </div>
  );
};

export const Cta: React.FC = () => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const btn = spring({frame: frame - 96, fps, config: {damping: 12, stiffness: 120}});
  const pulse = 0.5 + 0.5 * Math.sin(frame / 8);
  const meta = useFadeUp(118, 18, 12);
  const url = useFadeUp(134, 18, 12);
  return (
    <AbsoluteFill style={{padding: '130px 120px 70px', display: 'flex', flexDirection: 'column'}}>
      <div style={{display: 'flex', gap: 24}}>
        {STATS.map((s, i) => (
          <StatTile key={s.label} s={s} start={8 + i * 8} />
        ))}
      </div>

      <div style={{flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 34}}>
        <div style={{...useFadeUp(52, 18, 16), display: 'flex', alignItems: 'center', gap: 22}}>
          <Logo size={84} />
          <Wordmark size={42} />
        </div>
        <BlurWords lines={[{text: 'Start free.', accent: true}]} fontSize={132} start={62} align="center" />
        <div
          style={{
            opacity: btn,
            transform: `scale(${0.7 + 0.3 * btn})`,
            padding: '26px 64px',
            borderRadius: 999,
            fontSize: 40,
            fontWeight: 700,
            color: C.text,
            background: `linear-gradient(135deg, ${C.brandLogo}, ${C.brand})`,
            boxShadow: `0 0 ${50 + pulse * 40}px ${C.brand}88`,
          }}
        >
          {CONFIG.ctaLabel} →
        </div>
        <div style={{...meta, display: 'flex', gap: 16}}>
          <Pill color={C.brand}>~30s install</Pill>
          <Pill color={C.brand}>local-only by default</Pill>
          <Pill color={C.brand}>reversible: algo uninstall</Pill>
        </div>
        {CONFIG.showUrlInCta ? (
          <div style={{...url, fontFamily: F.mono, fontSize: 28, color: C.muted}}>{CONFIG.siteUrl}</div>
        ) : null}
      </div>
    </AbsoluteFill>
  );
};
