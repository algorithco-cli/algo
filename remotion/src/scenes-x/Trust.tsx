import React from 'react';
import {AbsoluteFill, interpolate, useCurrentFrame} from 'remotion';
import {C, F} from '../theme';
import {BlurWords, Card, clamp, useFadeUp} from '../ui';

const TILES = [
  {big: '<3ms', label: 'local decisions, p50'},
  {big: '13', label: 'versioned hard-deny rules'},
  {big: 'Shadow', label: 'mode first — nothing is blocked until you turn on enforce'},
  {big: 'Local', label: 'only by default — nothing leaves your machine'},
];

const Tile: React.FC<{big: string; label: string; start: number}> = ({big, label, start}) => {
  const frame = useCurrentFrame();
  const s = useFadeUp(start, 16, 24);
  const glow = interpolate(frame, [start + 6, start + 20], [0, 1], clamp);
  return (
    <div style={{...s, width: 456}}>
      <Card style={{padding: '30px 30px', height: 214, boxShadow: `0 0 ${40 * glow}px ${C.brand}33`}}>
        <div style={{fontFamily: F.mono, fontSize: 64, fontWeight: 700, color: C.brandStrong, letterSpacing: '-0.03em'}}>{big}</div>
        <div style={{fontSize: 25, lineHeight: 1.4, color: C.muted, marginTop: 8}}>{label}</div>
      </Card>
    </div>
  );
};

export const XTrust: React.FC = () => (
  <AbsoluteFill style={{padding: '84px 72px', justifyContent: 'center', gap: 44}}>
    <BlurWords lines={[{text: 'Safe to try.'}, {text: 'Easy to undo.', accent: true}]} fontSize={96} start={2} />
    <div style={{display: 'flex', flexWrap: 'wrap', gap: 24}}>
      {TILES.map((t, i) => (
        <Tile key={t.big} {...t} start={20 + i * 9} />
      ))}
    </div>
    <div style={{...useFadeUp(64, 16, 10), fontSize: 26, color: C.muted}}>
      Additive install · <span style={{fontFamily: F.mono, color: C.brandStrong}}>algo uninstall</span> restores your configs byte-identical.
    </div>
  </AbsoluteFill>
);
